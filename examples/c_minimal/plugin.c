extern void move_window(int, int, int);
extern void subscribe(int);

const int EVENT_KEY_PRESS_ID = 1;
const int EVENT_KEY_RELEASE_ID = 2;

const char id[] = "c_minimal";

void init(void) {
    // Subscribe to any key release
    subscribe(2);
}

void handle(void) {
    // Move window 2 on any event
    move_window(2, 300, 400);
}
