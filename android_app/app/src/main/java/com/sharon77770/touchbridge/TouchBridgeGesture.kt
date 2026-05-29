package com.sharon77770.touchbridge

import androidx.annotation.StringRes

enum class TouchBridgeGesture(
    val wireName: String,
    @StringRes val labelRes: Int,
    @StringRes val shortLabelRes: Int,
) {
    Tap("tap", R.string.gesture_tap, R.string.gesture_tap_short),
    DoubleTap("double_tap", R.string.gesture_double_tap, R.string.gesture_double_tap_short),
    LongPress("long_press", R.string.gesture_long_press, R.string.gesture_long_press_short),
    SwipeUp("swipe_up", R.string.gesture_swipe_up, R.string.gesture_swipe_up_short),
    SwipeDown("swipe_down", R.string.gesture_swipe_down, R.string.gesture_swipe_down_short),
    SwipeLeft("swipe_left", R.string.gesture_swipe_left, R.string.gesture_swipe_left_short),
    SwipeRight("swipe_right", R.string.gesture_swipe_right, R.string.gesture_swipe_right_short),
    TwoFingerTap("two_finger_tap", R.string.gesture_two_finger_tap, R.string.gesture_two_finger_tap_short),
    TwoFingerSwipeLeft(
        "two_finger_swipe_left",
        R.string.gesture_two_finger_swipe_left,
        R.string.gesture_two_finger_swipe_left_short,
    ),
    TwoFingerSwipeRight(
        "two_finger_swipe_right",
        R.string.gesture_two_finger_swipe_right,
        R.string.gesture_two_finger_swipe_right_short,
    ),
    ThreeFingerTap("three_finger_tap", R.string.gesture_three_finger_tap, R.string.gesture_three_finger_tap_short),
}
