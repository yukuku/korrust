package external;
public class Parent {
    public static String format(String a, int b) { return a + b; }
    public static String format(int a, int b) { return "sum" + (a + b); }
    private static int hidden() { return 19; }
}
