package dep;

public class Factory {
    public static <T extends CharSequence> T empty() { return null; }
    public static String describe(String value) { return value == null ? "null" : value; }
}
