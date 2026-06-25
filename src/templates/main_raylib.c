#include <raylib.h>

#define WINDOW_W 800
#define WINDOW_H 600
#define WINDOW_TITLE "Malware"

int main(void) {
    InitWindow(WINDOW_W, WINDOW_H, WINDOW_TITLE);
    SetTargetFPS(60);

    while (!WindowShouldClose()) {
        BeginDrawing();
        ClearBackground(BLACK);
        Vector2 text_size = MeasureTextEx(GetFontDefault(), "Ur mom", 30, 1.0f);
        DrawText("Ur mom", WINDOW_W / 2 - text_size.x / 2, WINDOW_H / 2 - text_size.y / 2, 30, WHITE);
        EndDrawing();
    }

    CloseWindow();
    return 0;
}