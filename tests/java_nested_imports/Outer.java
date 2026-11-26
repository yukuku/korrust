package nested.library;
public class Outer {
    public static class Item {
        public final int value;
        public Item(int value) { this.value = value; }
    }
    public enum Kind { A, B }
    public interface Box<T> { T get(); }
    public static class Middle {
        public static class Leaf { public static String text() { return "leaf"; } }
    }
    public class Inner {
        private final int value;
        public Inner(int value) { this.value = value; }
        public int number() { return value; }
    }
    static class PackageItem { public static int number() { return 7; } }
    protected static class ProtectedItem { public static int number() { return 11; } }
    private static class Secret { public static int number() { return 17; } }
    private static class HiddenOwner { public static class Exposed {} }
}
class PackageOuter {
    public static class Nested { public static int number() { return 13; } }
}
