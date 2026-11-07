package constructors;
public final class Effects {
    public static String log = "";
    public static int mark(int value) { log += value; return value; }
    public static int fail() { log += "X"; throw new IllegalStateException("default failed"); }
    public static void reset() { log = ""; }
}
