extern int subscribe(int*, unsigned int);
extern int debug_log(char*, unsigned int);

const int EVENT_KEY_PRESS_ID = 1;
const int EVENT_KEY_RELEASE_ID = 2;

const char id[] = "c_minimal";

void init(void) {
    // Subscribe to any key release
    int subscription[] = {EVENT_KEY_PRESS_ID, 64, 38};
    subscribe(subscription, sizeof(subscription) / 4);
}

void handle(void) {
    // Move window 2 on any event
    char message[] = "Win+A pressed";
    debug_log(message, sizeof(message) - 1);
}
