#include "cspot.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

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

int main(int argc, char **argv)
{
    const char *device_name = "Librespot Discovery Playback";
    const char *track_arg = NULL;
    char *track_uri = NULL;
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
    cspot_load_request_options_t *load_options = NULL;
    int exit_code = 0;

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
        track_uri = cspot_track_uri_from_input(track_arg, &error);
        if (!track_uri) {
            return report_error("invalid TRACK input", error);
        }
    }

    device_id = cspot_device_id_from_name(device_name, &error);
    if (!device_id) {
        exit_code = report_error("failed to compute device id", error);
        goto cleanup;
    }

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

    printf("Waiting for Spotify Connect credentials...\n");
    printf("Open Spotify and choose \"%s\" in the Connect list to authorize it.\n", device_name);

    cspot_discovery_next_result_t result =
        cspot_discovery_next(discovery, &credentials, &error);
    if (result != CSPOT_DISCOVERY_NEXT_CREDENTIALS) {
        if (result == CSPOT_DISCOVERY_NEXT_END) {
            exit_code = report_error("discovery stopped before credentials were received", error);
        } else {
            exit_code = report_error("failed to read discovery credentials", error);
        }
        goto cleanup;
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
    if (!cspot_connect_config_set_device_type(
            connect_config,
            CSPOT_DEVICE_TYPE_SPEAKER,
            &error)) {
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

    char *username = cspot_session_username(session);
    if (username) {
        printf("Connected as %s.\n", username);
        cspot_string_free(username);
    }

    printf("Spotify Connect ready.\n");

    if (track_uri) {
        load_options = cspot_load_request_options_create_default();
        if (!load_options) {
            exit_code = report_error("failed to create load options", error);
            goto cleanup;
        }
        if (!cspot_load_request_options_set_start_playing(load_options, true, &error)) {
            exit_code = report_error("failed to set load options", error);
            goto cleanup;
        }

        if (!cspot_spirc_activate(spirc, &error)) {
            exit_code = report_error("failed to activate Connect", error);
            goto cleanup;
        }

        const char *tracks[] = {track_uri};
        if (!cspot_spirc_load_tracks(spirc, tracks, 1, load_options, &error)) {
            exit_code = report_error("failed to load track", error);
            goto cleanup;
        }
        if (!cspot_spirc_play(spirc, &error)) {
            exit_code = report_error("failed to start playback", error);
            goto cleanup;
        }
    }

    if (!cspot_spirc_task_run(spirc_task, &error)) {
        exit_code = report_error("spirc task failed", error);
        goto cleanup;
    }

cleanup:
    if (load_options) {
        cspot_load_request_options_free(load_options);
    }
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
    if (track_uri) {
        cspot_string_free(track_uri);
    }

    return exit_code;
}
