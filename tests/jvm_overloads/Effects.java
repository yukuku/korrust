package overloads;

public final class Effects {
    public static String log = "";

    public static int mark(int value) {
        log += value + ";";
        return value;
    }

    public static int fail() {
        log += "fail;";
        throw new IllegalStateException("default");
    }
}
