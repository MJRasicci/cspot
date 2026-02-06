#include <jni.h>
#include <android/log.h>

extern "C" {
#include <cspot.h>
}

#include <algorithm>
#include <cstdint>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>

namespace {

constexpr const char *kLogTag = "cspot-android-client";
std::mutex g_android_context_mutex;
jobject g_android_context_global = nullptr;

int to_android_priority(cspot_log_level_t level) {
    switch (level) {
    case CSPOT_LOG_LEVEL_ERROR:
        return ANDROID_LOG_ERROR;
    case CSPOT_LOG_LEVEL_WARN:
        return ANDROID_LOG_WARN;
    case CSPOT_LOG_LEVEL_INFO:
        return ANDROID_LOG_INFO;
    case CSPOT_LOG_LEVEL_DEBUG:
        return ANDROID_LOG_DEBUG;
    case CSPOT_LOG_LEVEL_TRACE:
        return ANDROID_LOG_VERBOSE;
    case CSPOT_LOG_LEVEL_OFF:
    default:
        return ANDROID_LOG_DEFAULT;
    }
}

extern "C" void android_cspot_log_callback(
    const cspot_log_record_t *record,
    void * /*user_data*/) {
    if (record == nullptr || record->message == nullptr) {
        return;
    }

    const char *target = record->target ? record->target : "cspot";
    __android_log_print(
        to_android_priority(record->level),
        kLogTag,
        "[%s] %s",
        target,
        record->message);
}

void log_error(const std::string &message) {
    __android_log_print(ANDROID_LOG_ERROR, kLogTag, "%s", message.c_str());
}

std::string consume_error(cspot_error_t *error) {
    if (error == nullptr) {
        return {};
    }

    const char *message = cspot_error_message(error);
    std::string text = message ? message : "unknown cspot error";
    cspot_error_free(error);
    return text;
}

std::string copy_and_free_cspot_string(char *value) {
    if (value == nullptr) {
        return {};
    }
    std::string out(value);
    cspot_string_free(value);
    return out;
}

std::string json_escape(const std::string &value) {
    std::string escaped;
    escaped.reserve(value.size() + 8);

    for (unsigned char ch : value) {
        switch (ch) {
        case '\\':
            escaped += "\\\\";
            break;
        case '"':
            escaped += "\\\"";
            break;
        case '\n':
            escaped += "\\n";
            break;
        case '\r':
            escaped += "\\r";
            break;
        case '\t':
            escaped += "\\t";
            break;
        default:
            if (ch < 0x20) {
                static const char kHex[] = "0123456789ABCDEF";
                escaped += "\\u00";
                escaped += kHex[(ch >> 4) & 0x0F];
                escaped += kHex[ch & 0x0F];
            } else {
                escaped += static_cast<char>(ch);
            }
            break;
        }
    }

    return escaped;
}

std::string jstring_to_string(JNIEnv *env, jstring value) {
    if (env == nullptr || value == nullptr) {
        return {};
    }

    const char *chars = env->GetStringUTFChars(value, nullptr);
    if (chars == nullptr) {
        return {};
    }

    std::string text(chars);
    env->ReleaseStringUTFChars(value, chars);
    return text;
}

bool initialize_android_runtime_context(
    JNIEnv *env,
    jobject context,
    std::string *out_error) {
    if (out_error != nullptr) {
        out_error->clear();
    }

    if (env == nullptr) {
        if (out_error != nullptr) {
            *out_error = "JNI environment was null";
        }
        return false;
    }
    if (context == nullptr) {
        if (out_error != nullptr) {
            *out_error = "Android context argument was null";
        }
        return false;
    }

    JavaVM *java_vm = nullptr;
    if (env->GetJavaVM(&java_vm) != JNI_OK || java_vm == nullptr) {
        if (out_error != nullptr) {
            *out_error = "failed to resolve JavaVM";
        }
        return false;
    }

    jobject context_global = nullptr;
    {
        std::lock_guard<std::mutex> lock(g_android_context_mutex);
        if (g_android_context_global == nullptr) {
            g_android_context_global = env->NewGlobalRef(context);
            if (g_android_context_global == nullptr) {
                if (out_error != nullptr) {
                    *out_error = "failed to create global Android context reference";
                }
                return false;
            }
        }
        context_global = g_android_context_global;
    }

    cspot_error_t *error = nullptr;
    const bool ok = cspot_android_initialize_context(
        reinterpret_cast<void *>(java_vm),
        reinterpret_cast<void *>(context_global),
        &error);

    if (!ok) {
        std::string message = consume_error(error);
        if (message.empty()) {
            message = "failed to initialize Android audio context";
        }
        if (out_error != nullptr) {
            *out_error = message;
        }
        return false;
    }

    if (error != nullptr) {
        cspot_error_free(error);
    }

    return true;
}

class Engine {
  public:
    Engine() = default;

    void start(const std::string &device_name) {
        std::string normalized = device_name;
        if (normalized.empty()) {
            normalized = "cspot Android Client";
        }

        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (running_) {
                status_message_ = "cspot already running";
                return;
            }

            running_ = true;
            ready_ = false;
            device_name_ = normalized;
            status_message_ = "Initializing cspot runtime";
            last_error_.clear();
        }

        std::thread worker(&Engine::run, this, normalized);
        worker.detach();
    }

    std::string snapshot_json() {
        bool running = false;
        bool ready = false;
        bool connected = false;
        int playback_state = static_cast<int>(CSPOT_PLAYBACK_STATE_INVALID);
        uint32_t position_ms = 0;
        uint32_t duration_ms = 0;
        uint16_t volume = 0;

        std::string status_message;
        std::string device_name;
        std::string title;
        std::string artist;
        std::string album;
        std::string artwork_url;

        {
            std::lock_guard<std::mutex> lock(mutex_);

            running = running_;
            ready = ready_;
            status_message = status_message_;
            device_name = device_name_;

            if (spirc_ != nullptr) {
                connected = cspot_spirc_is_connected(spirc_);
                playback_state = static_cast<int>(cspot_spirc_playback_state(spirc_));
                position_ms = cspot_spirc_current_position_ms(spirc_);
                duration_ms = cspot_spirc_current_track_duration_ms(spirc_);
                volume = cspot_spirc_current_volume(spirc_);

                title = copy_and_free_cspot_string(cspot_spirc_current_track_title(spirc_));
                artist = copy_and_free_cspot_string(cspot_spirc_current_track_artist(spirc_));
                album = copy_and_free_cspot_string(cspot_spirc_current_track_album(spirc_));
                artwork_url =
                    copy_and_free_cspot_string(cspot_spirc_current_track_artwork_url(spirc_));
            }
        }

        std::ostringstream out;
        out << "{";
        out << "\"running\":" << (running ? "true" : "false") << ",";
        out << "\"ready\":" << (ready ? "true" : "false") << ",";
        out << "\"connected\":" << (connected ? "true" : "false") << ",";
        out << "\"playbackState\":" << playback_state << ",";
        out << "\"positionMs\":" << position_ms << ",";
        out << "\"durationMs\":" << duration_ms << ",";
        out << "\"volume\":" << volume << ",";
        out << "\"statusMessage\":\"" << json_escape(status_message) << "\",";
        out << "\"deviceName\":\"" << json_escape(device_name) << "\",";
        out << "\"title\":\"" << json_escape(title) << "\",";
        out << "\"artist\":\"" << json_escape(artist) << "\",";
        out << "\"album\":\"" << json_escape(album) << "\",";
        out << "\"artworkUrl\":\"" << json_escape(artwork_url) << "\"";
        out << "}";

        return out.str();
    }

    bool play_pause() { return run_simple_spirc_command(cspot_spirc_play_pause); }

    bool next() { return run_simple_spirc_command(cspot_spirc_next); }

    bool previous() { return run_simple_spirc_command(cspot_spirc_prev); }

    bool transfer() { return run_simple_spirc_command(cspot_spirc_transfer); }

    bool seek_to(uint32_t position_ms) {
        std::lock_guard<std::mutex> lock(mutex_);
        if (spirc_ == nullptr) {
            set_error_locked("Seek unavailable: Spotify Connect is not ready");
            return false;
        }

        cspot_error_t *error = nullptr;
        const bool ok = cspot_spirc_seek_to(spirc_, position_ms, &error);
        if (!ok) {
            std::string message = consume_error(error);
            if (message.empty()) {
                message = "Seek command failed";
            }
            set_error_locked(message);
            return false;
        }
        if (error != nullptr) {
            cspot_error_free(error);
        }
        return true;
    }

    bool set_volume(uint16_t volume) {
        std::lock_guard<std::mutex> lock(mutex_);
        if (spirc_ == nullptr) {
            set_error_locked("Volume unavailable: Spotify Connect is not ready");
            return false;
        }

        cspot_error_t *error = nullptr;
        const bool ok = cspot_spirc_set_volume(spirc_, volume, &error);
        if (!ok) {
            std::string message = consume_error(error);
            if (message.empty()) {
                message = "Volume command failed";
            }
            set_error_locked(message);
            return false;
        }
        if (error != nullptr) {
            cspot_error_free(error);
        }
        return true;
    }

    std::string take_last_error() {
        std::lock_guard<std::mutex> lock(mutex_);
        std::string message = last_error_;
        last_error_.clear();
        return message;
    }

    void report_initialization_error(const std::string &message) {
        std::lock_guard<std::mutex> lock(mutex_);
        running_ = false;
        ready_ = false;
        status_message_ = "cspot error: " + message;
        set_error_locked(message);
    }

  private:
    struct Handles {
        cspot_discovery_t *discovery = nullptr;
        cspot_credentials_t *credentials = nullptr;
        cspot_session_t *session = nullptr;
        cspot_mixer_t *mixer = nullptr;
        cspot_player_t *player = nullptr;
        cspot_connect_config_t *connect_config = nullptr;
        cspot_spirc_t *spirc = nullptr;
        cspot_spirc_task_t *spirc_task = nullptr;
    };

    bool run_simple_spirc_command(bool (*command)(const cspot_spirc_t *, cspot_error_t **)) {
        std::lock_guard<std::mutex> lock(mutex_);
        if (spirc_ == nullptr) {
            set_error_locked("Spotify Connect is not ready");
            return false;
        }

        cspot_error_t *error = nullptr;
        const bool ok = command(spirc_, &error);
        if (!ok) {
            std::string message = consume_error(error);
            if (message.empty()) {
                message = "Spotify Connect command failed";
            }
            set_error_locked(message);
            return false;
        }
        if (error != nullptr) {
            cspot_error_free(error);
        }
        return true;
    }

    void set_error_locked(const std::string &message) {
        if (!message.empty()) {
            last_error_ = message;
        }
    }

    Handles detach_handles_locked() {
        Handles handles;
        handles.discovery = discovery_;
        handles.credentials = credentials_;
        handles.session = session_;
        handles.mixer = mixer_;
        handles.player = player_;
        handles.connect_config = connect_config_;
        handles.spirc = spirc_;
        handles.spirc_task = spirc_task_;

        discovery_ = nullptr;
        credentials_ = nullptr;
        session_ = nullptr;
        mixer_ = nullptr;
        player_ = nullptr;
        connect_config_ = nullptr;
        spirc_ = nullptr;
        spirc_task_ = nullptr;
        ready_ = false;

        return handles;
    }

    static void free_handles(const Handles &handles) {
        if (handles.spirc_task != nullptr) {
            cspot_spirc_task_free(handles.spirc_task);
        }
        if (handles.spirc != nullptr) {
            cspot_spirc_free(handles.spirc);
        }
        if (handles.connect_config != nullptr) {
            cspot_connect_config_free(handles.connect_config);
        }
        if (handles.player != nullptr) {
            cspot_player_free(handles.player);
        }
        if (handles.mixer != nullptr) {
            cspot_mixer_free(handles.mixer);
        }
        if (handles.session != nullptr) {
            cspot_session_free(handles.session);
        }
        if (handles.credentials != nullptr) {
            cspot_credentials_free(handles.credentials);
        }
        if (handles.discovery != nullptr) {
            cspot_discovery_free(handles.discovery);
        }
    }

    void ensure_logging_initialized() {
        bool should_initialize = false;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (!log_initialized_) {
                log_initialized_ = true;
                should_initialize = true;
            }
        }

        if (!should_initialize) {
            return;
        }

        cspot_log_config_t config;
        cspot_log_config_init(&config);
        config.level = CSPOT_LOG_LEVEL_DEBUG;
        config.callback = android_cspot_log_callback;
        config.user_data = nullptr;

        cspot_error_t *error = nullptr;
        if (!cspot_log_init(&config, &error)) {
            std::string message = consume_error(error);
            if (message.empty()) {
                message = "failed to initialize cspot logging";
            }
            log_error(message);
            std::lock_guard<std::mutex> lock(mutex_);
            set_error_locked(message);
        }
    }

    void run(std::string device_name) {
        ensure_logging_initialized();

        cspot_error_t *error = nullptr;
        char *device_id = nullptr;

        std::string fatal_error;

        {
            std::lock_guard<std::mutex> lock(mutex_);
            status_message_ = "Calculating Spotify device id";
        }

        device_id = cspot_device_id_from_name(device_name.c_str(), &error);
        if (device_id == nullptr) {
            fatal_error = consume_error(error);
            if (fatal_error.empty()) {
                fatal_error = "failed to compute device id";
            }
            goto cleanup;
        }

        {
            std::lock_guard<std::mutex> lock(mutex_);
            status_message_ = "Starting Spotify Connect discovery";
        }

        {
            const char *client_id = cspot_session_default_client_id();
            if (client_id == nullptr) {
                fatal_error = "failed to read default Spotify client id";
                goto cleanup;
            }

            cspot_discovery_t *discovery = cspot_discovery_create(
                device_id,
                client_id,
                device_name.c_str(),
                CSPOT_DEVICE_TYPE_SMARTPHONE,
                &error);
            if (discovery == nullptr) {
                fatal_error = consume_error(error);
                if (fatal_error.empty()) {
                    fatal_error = "failed to start discovery service";
                }
                goto cleanup;
            }

            std::lock_guard<std::mutex> lock(mutex_);
            discovery_ = discovery;
            status_message_ = "Waiting for credentials. Select this device in Spotify Connect.";
        }

        {
            cspot_credentials_t *credentials = nullptr;
            cspot_discovery_t *discovery = nullptr;

            {
                std::lock_guard<std::mutex> lock(mutex_);
                discovery = discovery_;
            }

            if (discovery == nullptr) {
                fatal_error = "discovery handle was unavailable";
                goto cleanup;
            }

            const cspot_discovery_next_result_t result =
                cspot_discovery_next(discovery, &credentials, &error);

            {
                std::lock_guard<std::mutex> lock(mutex_);
                if (discovery_ == discovery) {
                    discovery_ = nullptr;
                }
            }
            cspot_discovery_free(discovery);

            if (result != CSPOT_DISCOVERY_NEXT_CREDENTIALS || credentials == nullptr) {
                if (result == CSPOT_DISCOVERY_NEXT_END) {
                    fatal_error = "discovery stopped before credentials were received";
                } else {
                    fatal_error = consume_error(error);
                    if (fatal_error.empty()) {
                        fatal_error = "failed to read discovery credentials";
                    }
                }
                goto cleanup;
            }

            std::lock_guard<std::mutex> lock(mutex_);
            credentials_ = credentials;
            status_message_ = "Credentials received, preparing playback session";
        }

        {
            cspot_session_t *session = cspot_session_create(device_id, &error);
            if (session == nullptr) {
                fatal_error = consume_error(error);
                if (fatal_error.empty()) {
                    fatal_error = "failed to create Spotify session";
                }
                goto cleanup;
            }
            std::lock_guard<std::mutex> lock(mutex_);
            session_ = session;
        }

        {
            cspot_mixer_t *mixer = cspot_mixer_create_default(&error);
            if (mixer == nullptr) {
                fatal_error = consume_error(error);
                if (fatal_error.empty()) {
                    fatal_error = "failed to create playback mixer";
                }
                goto cleanup;
            }
            std::lock_guard<std::mutex> lock(mutex_);
            mixer_ = mixer;
        }

        {
            cspot_player_t *player = nullptr;
            {
                std::lock_guard<std::mutex> lock(mutex_);
                player = cspot_player_create_default(session_, mixer_, &error);
            }
            if (player == nullptr) {
                fatal_error = consume_error(error);
                if (fatal_error.empty()) {
                    fatal_error = "failed to create player";
                }
                goto cleanup;
            }
            std::lock_guard<std::mutex> lock(mutex_);
            player_ = player;
        }

        {
            cspot_connect_config_t *config = cspot_connect_config_create_default();
            if (config == nullptr) {
                fatal_error = "failed to allocate connect config";
                goto cleanup;
            }
            std::lock_guard<std::mutex> lock(mutex_);
            connect_config_ = config;
        }

        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (!cspot_connect_config_set_name(connect_config_, device_name.c_str(), &error)) {
                fatal_error = consume_error(error);
                if (fatal_error.empty()) {
                    fatal_error = "failed to set connect device name";
                }
                goto cleanup;
            }

            if (!cspot_connect_config_set_device_type(
                    connect_config_, CSPOT_DEVICE_TYPE_SMARTPHONE, &error)) {
                fatal_error = consume_error(error);
                if (fatal_error.empty()) {
                    fatal_error = "failed to set connect device type";
                }
                goto cleanup;
            }

            status_message_ = "Starting Spotify Connect runtime";
        }

        {
            cspot_spirc_task_t *spirc_task = nullptr;
            cspot_spirc_t *spirc = nullptr;

            {
                std::lock_guard<std::mutex> lock(mutex_);
                spirc = cspot_spirc_create(
                    connect_config_, session_, credentials_, player_, mixer_, &spirc_task, &error);
            }

            if (spirc == nullptr || spirc_task == nullptr) {
                fatal_error = consume_error(error);
                if (fatal_error.empty()) {
                    fatal_error = "failed to create Spotify Connect runtime";
                }
                if (spirc_task != nullptr) {
                    cspot_spirc_task_free(spirc_task);
                }
                if (spirc != nullptr) {
                    cspot_spirc_free(spirc);
                }
                goto cleanup;
            }

            {
                std::lock_guard<std::mutex> lock(mutex_);
                spirc_ = spirc;
                spirc_task_ = spirc_task;
                ready_ = true;
                status_message_ = "Spotify Connect ready";
            }
        }

        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (!cspot_spirc_transfer(spirc_, &error)) {
                std::string warning = consume_error(error);
                if (!warning.empty()) {
                    set_error_locked(warning);
                    log_error(warning);
                }
            } else if (error != nullptr) {
                cspot_error_free(error);
            }
        }

        {
            cspot_spirc_task_t *spirc_task = nullptr;
            {
                std::lock_guard<std::mutex> lock(mutex_);
                spirc_task = spirc_task_;
            }

            if (spirc_task == nullptr) {
                fatal_error = "spirc task handle was unavailable";
                goto cleanup;
            }

            if (!cspot_spirc_task_run(spirc_task, &error)) {
                fatal_error = consume_error(error);
                if (fatal_error.empty()) {
                    fatal_error = "Spotify Connect runtime stopped unexpectedly";
                }
                goto cleanup;
            }
        }

        {
            std::lock_guard<std::mutex> lock(mutex_);
            status_message_ = "Spotify Connect session ended";
        }

    cleanup:
        if (device_id != nullptr) {
            cspot_string_free(device_id);
        }

        Handles handles;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            handles = detach_handles_locked();
            running_ = false;
            ready_ = false;
            if (!fatal_error.empty()) {
                status_message_ = "cspot error: " + fatal_error;
                set_error_locked(fatal_error);
                log_error(fatal_error);
            }
        }
        free_handles(handles);
    }

    std::mutex mutex_;

    bool running_ = false;
    bool ready_ = false;
    bool log_initialized_ = false;

    std::string status_message_ = "Idle";
    std::string last_error_;
    std::string device_name_ = "cspot Android Client";

    cspot_discovery_t *discovery_ = nullptr;
    cspot_credentials_t *credentials_ = nullptr;
    cspot_session_t *session_ = nullptr;
    cspot_mixer_t *mixer_ = nullptr;
    cspot_player_t *player_ = nullptr;
    cspot_connect_config_t *connect_config_ = nullptr;
    cspot_spirc_t *spirc_ = nullptr;
    cspot_spirc_task_t *spirc_task_ = nullptr;
};

Engine g_engine;

} // namespace

extern "C" JNIEXPORT void JNICALL
Java_io_cspot_androidclient_NativeBridge_nativeStart(
    JNIEnv *env,
    jclass,
    jstring device_name,
    jobject context) {
    std::string init_error;
    if (!initialize_android_runtime_context(env, context, &init_error)) {
        if (init_error.empty()) {
            init_error = "failed to initialize Android runtime context";
        }
        g_engine.report_initialization_error(init_error);
        log_error(init_error);
        return;
    }
    g_engine.start(jstring_to_string(env, device_name));
}

extern "C" JNIEXPORT jstring JNICALL
Java_io_cspot_androidclient_NativeBridge_nativeGetSnapshotJson(JNIEnv *env, jclass) {
    std::string snapshot = g_engine.snapshot_json();
    return env->NewStringUTF(snapshot.c_str());
}

extern "C" JNIEXPORT jboolean JNICALL
Java_io_cspot_androidclient_NativeBridge_nativePlayPause(JNIEnv *, jclass) {
    return g_engine.play_pause() ? JNI_TRUE : JNI_FALSE;
}

extern "C" JNIEXPORT jboolean JNICALL
Java_io_cspot_androidclient_NativeBridge_nativeNext(JNIEnv *, jclass) {
    return g_engine.next() ? JNI_TRUE : JNI_FALSE;
}

extern "C" JNIEXPORT jboolean JNICALL
Java_io_cspot_androidclient_NativeBridge_nativePrevious(JNIEnv *, jclass) {
    return g_engine.previous() ? JNI_TRUE : JNI_FALSE;
}

extern "C" JNIEXPORT jboolean JNICALL
Java_io_cspot_androidclient_NativeBridge_nativeTransfer(JNIEnv *, jclass) {
    return g_engine.transfer() ? JNI_TRUE : JNI_FALSE;
}

extern "C" JNIEXPORT jboolean JNICALL
Java_io_cspot_androidclient_NativeBridge_nativeSeekTo(JNIEnv *, jclass, jint position_ms) {
    uint32_t safe_position = 0;
    if (position_ms > 0) {
        safe_position = static_cast<uint32_t>(position_ms);
    }
    return g_engine.seek_to(safe_position) ? JNI_TRUE : JNI_FALSE;
}

extern "C" JNIEXPORT jboolean JNICALL
Java_io_cspot_androidclient_NativeBridge_nativeSetVolume(JNIEnv *, jclass, jint volume) {
    int clamped = std::clamp(static_cast<int>(volume), 0, 65535);
    return g_engine.set_volume(static_cast<uint16_t>(clamped)) ? JNI_TRUE : JNI_FALSE;
}

extern "C" JNIEXPORT jstring JNICALL
Java_io_cspot_androidclient_NativeBridge_nativeTakeLastError(JNIEnv *env, jclass) {
    std::string message = g_engine.take_last_error();
    return env->NewStringUTF(message.c_str());
}
