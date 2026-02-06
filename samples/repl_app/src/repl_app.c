#include "cspot.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <pthread.h>
#endif

typedef struct spirc_runner_t {
    cspot_spirc_task_t *task;
    int failed;
    int completed;
    char *error_message;
} spirc_runner_t;

static char *copy_string(const char *value)
{
    if (!value) {
        return NULL;
    }
    size_t len = strlen(value);
    char *copy = (char *)malloc(len + 1);
    if (!copy) {
        return NULL;
    }
    memcpy(copy, value, len + 1);
    return copy;
}

static int report_error(const char *context, cspot_error_t *error)
{
    const char *message = error ? cspot_error_message(error) : NULL;
    fprintf(stderr, "%s: %s\n", context, message ? message : "unknown error");
    cspot_error_free(error);
    return 1;
}

static void print_usage(const char *program)
{
    fprintf(stderr, "Usage: %s [TRACK]\n", program);
    fprintf(stderr, "TRACK can be a Spotify URI (spotify:track:...) or a base62 track id.\n");
}

static const char *playback_state_name(cspot_playback_state_t state)
{
    switch (state) {
    case CSPOT_PLAYBACK_STATE_STOPPED:
        return "stopped";
    case CSPOT_PLAYBACK_STATE_LOADING:
        return "loading";
    case CSPOT_PLAYBACK_STATE_PLAYING:
        return "playing";
    case CSPOT_PLAYBACK_STATE_PAUSED:
        return "paused";
    default:
        return "invalid";
    }
}

static bool parse_on_off(const char *text, bool *value)
{
    if (!text || !value) {
        return false;
    }
    if (strcmp(text, "on") == 0 || strcmp(text, "1") == 0 || strcmp(text, "true") == 0) {
        *value = true;
        return true;
    }
    if (strcmp(text, "off") == 0 || strcmp(text, "0") == 0 || strcmp(text, "false") == 0) {
        *value = false;
        return true;
    }
    return false;
}

static bool parse_u32(const char *text, uint32_t *value)
{
    unsigned long parsed = 0;
    char *end = NULL;

    if (!text || !value || *text == '\0') {
        return false;
    }

    parsed = strtoul(text, &end, 10);
    if (!end || *end != '\0' || parsed > UINT32_MAX) {
        return false;
    }

    *value = (uint32_t)parsed;
    return true;
}

static bool parse_u16(const char *text, uint16_t *value)
{
    uint32_t parsed = 0;
    if (!parse_u32(text, &parsed) || parsed > UINT16_MAX) {
        return false;
    }
    *value = (uint16_t)parsed;
    return true;
}

static bool load_and_play_track(
    cspot_spirc_t *spirc,
    const char *track_input,
    cspot_error_t **error)
{
    bool ok = false;
    char *track_uri = NULL;
    cspot_load_request_options_t *options = NULL;

    track_uri = cspot_track_uri_from_input(track_input, error);
    if (!track_uri) {
        return false;
    }

    options = cspot_load_request_options_create_default();
    if (!options) {
        cspot_string_free(track_uri);
        return false;
    }

    if (!cspot_spirc_activate(spirc, error)) {
        goto cleanup;
    }

    if (!cspot_load_request_options_set_start_playing(options, true, error)) {
        goto cleanup;
    }

    {
        const char *tracks[] = {track_uri};
        if (!cspot_spirc_load_tracks(spirc, tracks, 1, options, error)) {
            goto cleanup;
        }
    }

    ok = true;

cleanup:
    cspot_load_request_options_free(options);
    cspot_string_free(track_uri);
    return ok;
}

static void print_status(cspot_spirc_t *spirc)
{
    bool connected = cspot_spirc_is_connected(spirc);
    cspot_playback_state_t state = cspot_spirc_playback_state(spirc);
    uint32_t position_ms = cspot_spirc_current_position_ms(spirc);
    uint32_t duration_ms = cspot_spirc_current_track_duration_ms(spirc);
    uint16_t volume = cspot_spirc_current_volume(spirc);

    char *track_id = cspot_spirc_current_track_id(spirc);
    char *track_uri = cspot_spirc_current_track_uri(spirc);
    char *artist = cspot_spirc_current_track_artist(spirc);
    char *album = cspot_spirc_current_track_album(spirc);
    char *title = cspot_spirc_current_track_title(spirc);
    char *artwork_url = cspot_spirc_current_track_artwork_url(spirc);

    printf(
        "connected=%s state=%s pos=%u/%u ms volume=%u shuffle=%s repeat=%s repeat_track=%s\n",
        connected ? "yes" : "no",
        playback_state_name(state),
        position_ms,
        duration_ms,
        volume,
        cspot_spirc_is_shuffle_enabled(spirc) ? "on" : "off",
        cspot_spirc_is_repeat_context_enabled(spirc) ? "on" : "off",
        cspot_spirc_is_repeat_track_enabled(spirc) ? "on" : "off");

    printf(
        "track: id=%s title=%s artist=%s album=%s\n",
        track_id ? track_id : "(none)",
        title ? title : "(none)",
        artist ? artist : "(none)",
        album ? album : "(none)");

    if (track_uri) {
        printf("uri: %s\n", track_uri);
    }
    if (artwork_url) {
        printf("artwork: %s\n", artwork_url);
    }

    cspot_string_free(track_id);
    cspot_string_free(track_uri);
    cspot_string_free(artist);
    cspot_string_free(album);
    cspot_string_free(title);
    cspot_string_free(artwork_url);
}

static void print_help(void)
{
    puts("Commands:");
    puts("  help");
    puts("  status");
    puts("  activate");
    puts("  transfer");
    puts("  play");
    puts("  pause");
    puts("  toggle");
    puts("  next");
    puts("  prev");
    puts("  seek <ms>");
    puts("  volume <0-65535>");
    puts("  volup");
    puts("  voldown");
    puts("  shuffle <on|off>");
    puts("  repeat <on|off>");
    puts("  repeat-track <on|off>");
    puts("  load <track-uri-or-base62-id>");
    puts("  queue <spotify-uri>");
    puts("  disconnect");
    puts("  quit");
}

#ifdef _WIN32
static DWORD WINAPI spirc_runner_main(LPVOID arg)
#else
static void *spirc_runner_main(void *arg)
#endif
{
    spirc_runner_t *runner = (spirc_runner_t *)arg;
    cspot_error_t *error = NULL;

    if (!runner) {
#ifdef _WIN32
        return 0;
#else
        return NULL;
#endif
    }

    if (!cspot_spirc_task_run(runner->task, &error)) {
        runner->failed = 1;
        runner->error_message = copy_string(
            error ? cspot_error_message(error) : "unknown error while running spirc task");
        cspot_error_free(error);
    }

    runner->completed = 1;

#ifdef _WIN32
    return 0;
#else
    return NULL;
#endif
}

int main(int argc, char **argv)
{
    const char *device_name = "Librespot REPL";
    const char *track_arg = NULL;

    char *device_id = NULL;

    cspot_error_t *error = NULL;

    cspot_discovery_t *discovery = NULL;
    cspot_credentials_t *credentials = NULL;
    cspot_session_t *session = NULL;
    cspot_mixer_t *mixer = NULL;
    cspot_player_t *player = NULL;
    cspot_connect_config_t *connect_config = NULL;
    cspot_spirc_t *spirc = NULL;
    cspot_spirc_task_t *spirc_task = NULL;

    spirc_runner_t runner;
    int runner_started = 0;

#ifdef _WIN32
    HANDLE runner_thread = NULL;
#else
    pthread_t runner_thread;
#endif

    int exit_code = 0;

    memset(&runner, 0, sizeof(runner));

    if (!cspot_log_init(NULL, &error)) {
        report_error("failed to initialize logging", error);
        error = NULL;
    }

    if (argc > 2) {
        print_usage(argv[0]);
        return 1;
    }
    if (argc == 2) {
        track_arg = argv[1];
    }

    device_id = cspot_device_id_from_name(device_name, &error);
    if (!device_id) {
        exit_code = report_error("failed to compute device id", error);
        goto cleanup;
    }

    {
        const char *client_id = cspot_session_default_client_id();
        if (!client_id) {
            exit_code = report_error("failed to read default client id", error);
            goto cleanup;
        }

        discovery = cspot_discovery_create(
            device_id,
            client_id,
            device_name,
            CSPOT_DEVICE_TYPE_SPEAKER,
            &error);
        if (!discovery) {
            exit_code = report_error("failed to start discovery", error);
            goto cleanup;
        }
    }

    printf("Waiting for Spotify Connect credentials...\n");
    printf("Open Spotify and choose \"%s\" in the Connect list to authorize it.\n", device_name);

    {
        cspot_discovery_next_result_t result =
            cspot_discovery_next(discovery, &credentials, &error);
        if (result != CSPOT_DISCOVERY_NEXT_CREDENTIALS) {
            if (result == CSPOT_DISCOVERY_NEXT_END) {
                exit_code = report_error(
                    "discovery stopped before credentials were received",
                    error);
            } else {
                exit_code = report_error("failed to read discovery credentials", error);
            }
            goto cleanup;
        }
    }

    session = cspot_session_create(device_id, &error);
    if (!session) {
        exit_code = report_error("failed to create session", error);
        goto cleanup;
    }

    mixer = cspot_mixer_create_default(&error);
    if (!mixer) {
        exit_code = report_error("failed to initialize mixer", error);
        goto cleanup;
    }

    player = cspot_player_create_default(session, mixer, &error);
    if (!player) {
        exit_code = report_error("failed to initialize player", error);
        goto cleanup;
    }

    connect_config = cspot_connect_config_create_default();
    if (!connect_config) {
        exit_code = report_error("failed to create connect config", error);
        goto cleanup;
    }

    if (!cspot_connect_config_set_name(connect_config, device_name, &error)) {
        exit_code = report_error("failed to set connect name", error);
        goto cleanup;
    }

    if (!cspot_connect_config_set_device_type(connect_config, CSPOT_DEVICE_TYPE_SPEAKER, &error)) {
        exit_code = report_error("failed to set connect device type", error);
        goto cleanup;
    }

    printf("Starting Spotify Connect...\n");
    spirc = cspot_spirc_create(
        connect_config,
        session,
        credentials,
        player,
        mixer,
        &spirc_task,
        &error);
    if (!spirc) {
        exit_code = report_error("failed to start Connect", error);
        goto cleanup;
    }

    runner.task = spirc_task;

#ifdef _WIN32
    runner_thread = CreateThread(NULL, 0, spirc_runner_main, &runner, 0, NULL);
    if (!runner_thread) {
        fprintf(stderr, "failed to start spirc thread\n");
        exit_code = 1;
        goto cleanup;
    }
    runner_started = 1;
#else
    if (pthread_create(&runner_thread, NULL, spirc_runner_main, &runner) != 0) {
        fprintf(stderr, "failed to start spirc thread\n");
        exit_code = 1;
        goto cleanup;
    }
    runner_started = 1;
#endif

    if (!cspot_spirc_transfer(spirc, &error)) {
        report_error("initial transfer attempt failed", error);
        error = NULL;
    }

    {
        char *username = cspot_session_username(session);
        if (username) {
            printf("Connected as %s.\n", username);
            cspot_string_free(username);
        }
    }

    if (track_arg) {
        if (!load_and_play_track(spirc, track_arg, &error)) {
            exit_code = report_error("failed to load initial track", error);
            goto cleanup;
        }
    }

    print_help();
    puts("Tip: use `load <track>` to start local playback, or `transfer` to pull playback from another Spotify client.");

    for (;;) {
        char line[1024];
        char *cmd = NULL;
        char *arg = NULL;

        if (runner.completed) {
            break;
        }

        printf("cspot> ");
        fflush(stdout);

        if (!fgets(line, sizeof(line), stdin)) {
            break;
        }

        cmd = strtok(line, " \t\r\n");
        if (!cmd) {
            continue;
        }
        arg = strtok(NULL, " \t\r\n");

        if (strcmp(cmd, "help") == 0) {
            print_help();
            continue;
        }

        if (strcmp(cmd, "quit") == 0 || strcmp(cmd, "exit") == 0) {
            break;
        }

        if (strcmp(cmd, "status") == 0) {
            print_status(spirc);
            continue;
        }

        if (strcmp(cmd, "activate") == 0) {
            if (!cspot_spirc_activate(spirc, &error)) {
                report_error("activate failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "transfer") == 0) {
            if (!cspot_spirc_transfer(spirc, &error)) {
                report_error("transfer failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "play") == 0) {
            char *track_id = cspot_spirc_current_track_id(spirc);
            if (!track_id || track_id[0] == '\0') {
                puts("No track is loaded yet. Use `load <track>` or `transfer` first.");
                cspot_string_free(track_id);
                continue;
            }
            cspot_string_free(track_id);
            if (!cspot_spirc_resume(spirc, &error)) {
                report_error("play failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "pause") == 0) {
            if (!cspot_spirc_pause(spirc, &error)) {
                report_error("pause failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "toggle") == 0) {
            if (!cspot_spirc_play_pause(spirc, &error)) {
                report_error("toggle failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "next") == 0) {
            if (!cspot_spirc_next(spirc, &error)) {
                report_error("next failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "prev") == 0) {
            if (!cspot_spirc_prev(spirc, &error)) {
                report_error("prev failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "seek") == 0) {
            uint32_t position_ms = 0;
            if (!parse_u32(arg, &position_ms)) {
                puts("usage: seek <ms>");
                continue;
            }
            if (!cspot_spirc_seek_to(spirc, position_ms, &error)) {
                report_error("seek failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "volume") == 0) {
            uint16_t volume = 0;
            if (!parse_u16(arg, &volume)) {
                puts("usage: volume <0-65535>");
                continue;
            }
            if (!cspot_spirc_set_volume(spirc, volume, &error)) {
                report_error("volume failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "volup") == 0) {
            if (!cspot_spirc_volume_up(spirc, &error)) {
                report_error("volup failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "voldown") == 0) {
            if (!cspot_spirc_volume_down(spirc, &error)) {
                report_error("voldown failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "shuffle") == 0 || strcmp(cmd, "repeat") == 0 || strcmp(cmd, "repeat-track") == 0) {
            bool enabled = false;
            bool ok = false;

            if (!parse_on_off(arg, &enabled)) {
                printf("usage: %s <on|off>\n", cmd);
                continue;
            }

            if (strcmp(cmd, "shuffle") == 0) {
                ok = cspot_spirc_set_shuffle(spirc, enabled, &error);
            } else if (strcmp(cmd, "repeat") == 0) {
                ok = cspot_spirc_set_repeat_context(spirc, enabled, &error);
            } else {
                ok = cspot_spirc_set_repeat_track(spirc, enabled, &error);
            }

            if (!ok) {
                report_error("set option failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "load") == 0) {
            if (!arg) {
                puts("usage: load <track-uri-or-base62-id>");
                continue;
            }
            if (!load_and_play_track(spirc, arg, &error)) {
                report_error("load failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "queue") == 0) {
            if (!arg) {
                puts("usage: queue <spotify-uri>");
                continue;
            }
            if (!cspot_spirc_add_to_queue(spirc, arg, &error)) {
                report_error("queue failed", error);
                error = NULL;
            }
            continue;
        }

        if (strcmp(cmd, "disconnect") == 0) {
            if (!cspot_spirc_disconnect(spirc, true, &error)) {
                report_error("disconnect failed", error);
                error = NULL;
            }
            continue;
        }

        printf("unknown command: %s\n", cmd);
    }

cleanup:
    if (spirc && runner_started) {
        cspot_error_t *shutdown_error = NULL;
        if (!cspot_spirc_shutdown(spirc, &shutdown_error)) {
            report_error("shutdown failed", shutdown_error);
        }
    }

    if (runner_started) {
#ifdef _WIN32
        WaitForSingleObject(runner_thread, INFINITE);
        CloseHandle(runner_thread);
#else
        pthread_join(runner_thread, NULL);
#endif
    }

    if (runner.failed) {
        fprintf(stderr, "spirc task error: %s\n", runner.error_message ? runner.error_message : "unknown");
        if (exit_code == 0) {
            exit_code = 1;
        }
    }

    free(runner.error_message);

    if (spirc_task) {
        cspot_spirc_task_free(spirc_task);
    }
    if (spirc) {
        cspot_spirc_free(spirc);
    }
    if (connect_config) {
        cspot_connect_config_free(connect_config);
    }
    if (player) {
        cspot_player_free(player);
    }
    if (mixer) {
        cspot_mixer_free(mixer);
    }
    if (session) {
        cspot_session_free(session);
    }
    if (credentials) {
        cspot_credentials_free(credentials);
    }
    if (discovery) {
        cspot_discovery_free(discovery);
    }
    if (device_id) {
        cspot_string_free(device_id);
    }

    return exit_code;
}
