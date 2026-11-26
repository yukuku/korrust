package star.library;
public class First {
    public static final int TOKEN = 13;
    public static int state = 3;
    public static int choose(int value) { return value + 10; }
    public static String choose(String value) { return "first:" + value; }
    public static int left(int value) { return value + 17; }
    public static int clash(int value) { return value; }
    public int instance(int value) { return value; }
    private static int hidden(int value) { return value; }
    protected static int guarded(int value) { return value; }
    static int packageOnly(int value) { return value; }
    public static class Nested { public int answer() { return 29; } }
}
