package io.cspot.androidclient;

import android.content.Context;

public final class NativeBridge {
    static {
        System.loadLibrary("cspot");
        System.loadLibrary("android_client_jni");
    }

    private NativeBridge() {
    }

    public static void start(String deviceName, Context context) {
        nativeStart(deviceName, context);
    }

    public static String snapshotJson() {
        return nativeGetSnapshotJson();
    }

    public static boolean playPause() {
        return nativePlayPause();
    }

    public static boolean next() {
        return nativeNext();
    }

    public static boolean previous() {
        return nativePrevious();
    }

    public static boolean transfer() {
        return nativeTransfer();
    }

    public static boolean seekTo(int positionMs) {
        return nativeSeekTo(positionMs);
    }

    public static boolean setVolume(int volume) {
        return nativeSetVolume(volume);
    }

    public static String takeLastError() {
        return nativeTakeLastError();
    }

    private static native void nativeStart(String deviceName, Context context);

    private static native String nativeGetSnapshotJson();

    private static native boolean nativePlayPause();

    private static native boolean nativeNext();

    private static native boolean nativePrevious();

    private static native boolean nativeTransfer();

    private static native boolean nativeSeekTo(int positionMs);

    private static native boolean nativeSetVolume(int volume);

    private static native String nativeTakeLastError();
}
