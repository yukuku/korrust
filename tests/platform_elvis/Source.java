package api;
public class Source {
    public static String fallback;
    public static int reads;
    public static String next() { ++reads; return fallback; }
}
