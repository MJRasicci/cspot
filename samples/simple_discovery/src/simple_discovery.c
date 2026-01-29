#include "cspot.h"

#include <stdint.h>
#include <stdio.h>

static int report_error(const char *context, cspot_error_t *error)
{
    const char *message = error ? cspot_error_message(error) : NULL;
    fprintf(stderr, "%s: %s\n", context, message ? message : "unknown error");
    cspot_error_free(error);
    return 1;
}

int main(void)
{
    const char *name = "Librespot";
    cspot_error_t *error = NULL;

    char *device_id = cspot_device_id_from_name(name, &error);
    if (!device_id) {
        return report_error("failed to compute device id", error);
    }

    const char *client_id = cspot_session_default_client_id();
    if (!client_id) {
        cspot_string_free(device_id);
        return report_error("failed to read default client id", error);
    }

    cspot_discovery_t *discovery =
        cspot_discovery_create(device_id, client_id, name, CSPOT_DEVICE_TYPE_COMPUTER, &error);
    cspot_string_free(device_id);
    if (!discovery) {
        return report_error("failed to start discovery", error);
    }

    for (;;) {
        cspot_credentials_t *credentials = NULL;
        cspot_discovery_next_result_t result =
            cspot_discovery_next(discovery, &credentials, &error);
        if (result == CSPOT_DISCOVERY_NEXT_CREDENTIALS) {
            const char *username = cspot_credentials_username(credentials);
            cspot_auth_type_t auth_type = cspot_credentials_auth_type(credentials);
            size_t auth_data_len = 0;
            const uint8_t *auth_data =
                cspot_credentials_auth_data(credentials, &auth_data_len);

            printf(
                "Received credentials: username=%s auth_type=%s auth_data_len=%zu\n",
                username ? username : "(none)",
                cspot_auth_type_name(auth_type),
                auth_data_len);

            (void)auth_data;
            cspot_credentials_free(credentials);
        } else if (result == CSPOT_DISCOVERY_NEXT_END) {
            break;
        } else {
            report_error("discovery stopped", error);
            break;
        }
    }

    cspot_discovery_free(discovery);
    return 0;
}
