package aliases.library;
public class JavaTypes {
    private static String trace = "";
    public static void clear() { trace = ""; }
    public static void mark(String value) { trace += value; }
    public static String argument(String value) { mark(value); return value; }
    public static String trace() { return trace; }
    public static class Pair<A, B> {
        public final A first;
        public final B second;
        public Pair(A first, B second) { this.first = first; this.second = second; }
    }
    public static class Box<T> {
        public final T value;
        public Box(T value) { this.value = value; }
    }
    public static class Bounded<A extends CharSequence> {
        public final A value;
        public Bounded(A value) { this.value = value; }
    }
    public static class Shadow<A> {
        public final String value;
        public <T extends CharSequence> Shadow(T value) { this.value = value.toString(); }
    }
    public static class Dependent<A> {
        public final A value;
        public <T extends A> Dependent(T value) { this.value = value; }
    }
}
