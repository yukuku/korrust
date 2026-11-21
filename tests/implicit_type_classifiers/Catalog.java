package classifierprobe;
public class Catalog {
    public static class Entry<T> {
        public String key;
        public T value;
        public Entry(String key, T value) { this.key = key; this.value = value; }
    }
    public enum Mode { FIRST, SECOND }
    private static class Hidden {}
    protected static class Guarded {}
}
