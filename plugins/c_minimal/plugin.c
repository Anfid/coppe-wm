#include <stdint.h>

extern int32_t subscribe(uint8_t*, uint32_t);
extern int32_t debug_log(char*, uint32_t);
extern int32_t event_read(uint8_t* ptr, uint32_t len, uint32_t offset);
extern int32_t event_len();

const uint32_t EVENT_KEY_PRESS_ID = 1;
const uint32_t EVENT_KEY_RELEASE_ID = 2;

void int32_to_le(int32_t i, uint8_t* buf) {
    buf[0] = (uint8_t)(i & 0x000000ff);
    buf[1] = (uint8_t)(i & 0x0000ff00);
    buf[2] = (uint8_t)(i & 0x00ff0000);
    buf[3] = (uint8_t)(i & 0xff000000);
}

int32_t le_to_int32(uint8_t* buf) {
    int32_t i = 0;

    i += buf[0];
    i += buf[1] << 8;
    i += buf[2] << 16;
    i += buf[3] << 24;

    return i;
}

void int16_to_le(int16_t i, uint8_t* buf) {
    buf[0] = (uint8_t)(i & 0x00ff);
    buf[1] = (uint8_t)(i & 0xff00);
}

void init(void) {
    // Initialize Win+A subscription on key press
    uint8_t subscription[7];
    int32_to_le(EVENT_KEY_PRESS_ID, subscription);
    int16_to_le(64, subscription + 4);
    subscription[6] = 38;

    subscribe(subscription, sizeof(subscription));

    // Reinitialize Win+A subscription on key release
    int32_to_le(EVENT_KEY_RELEASE_ID, subscription);

    subscribe(subscription, sizeof(subscription));
}

void handle(void) {
    // Key press and release events have size 7
    uint8_t buffer[7];
    event_read(buffer, 7, 0);

    int32_t id = le_to_int32(buffer);

    switch (id) {
        case EVENT_KEY_PRESS_ID: {
            char message[] = "Win+A pressed";
            debug_log(message, sizeof(message) - 1);
            break;
        }

        case EVENT_KEY_RELEASE_ID: {
            char message[] = "Win+A released";
            debug_log(message, sizeof(message) - 1);
            break;
        }
    }
}
