package singleton.library;
public class Events {
    private static String trace = "";
    public static void add(String value) { trace += value; }
    public static String trace() { return trace; }
    public static void clear() { trace = ""; }
    public static String receiver() { add("R"); return "r"; }
    public static String argument() { add("A"); return "a"; }
}
