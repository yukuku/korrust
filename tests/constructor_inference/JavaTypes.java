package constructors.library;
import java.util.Collection;
import java.util.List;
public class JavaTypes {
    private static String trace = "";
    public static void clear() { trace = ""; }
    public static void mark(String value) { trace += value; }
    public static String argument(String value) { mark(value); return value; }
    public static String trace() { return trace; }
    public static class Pair<A, B> {
        public Pair(A first, B second) { mark("J"); }
    }
    public static class Box<T> {
        private final T value;
        public Box(T value) { this.value = value; }
        public T getValue() { return value; }
    }
    public static class Bounded<T extends CharSequence> {
        private final T value;
        public Bounded(T value) { this.value = value; }
        public T getValue() { return value; }
    }
    public static class FromCollection<T> {
        private final Collection<? extends T> values;
        public FromCollection(Collection<? extends T> values) { this.values = values; }
        public T first() { return values.iterator().next(); }
    }
    public static class Choice<T> {
        public final int tag;
        public Choice(T value) { tag = 1; }
        public Choice(List<T> values) { tag = 2; }
    }
    public static class Base<A, B> {}
    public static class Flipped<X extends CharSequence, Y extends Number> extends Base<Y, X> {
        public Flipped() {}
    }
    public static class Shadow<T> {
        public final String text;
        public <T extends CharSequence> Shadow(T value) { text = value.toString(); }
    }
    public static class Dependent<T> {
        private final T value;
        public <U extends T> Dependent(U value) { this.value = value; }
        public T getValue() { return value; }
    }
    public static class Batch<T> {
        private final T[] values;
        @SafeVarargs public Batch(T... values) { this.values = values; }
        public T first() { return values[0]; }
    }
}
