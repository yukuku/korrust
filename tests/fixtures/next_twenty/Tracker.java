package next.support;

import java.util.List;

public final class Tracker {
    public static int calls;
    public static String events;
    public static void reset() { calls = 0; events = ""; }
    public static int bound(int value) { calls++; events += value; return value; }
    public static int[] array(int[] value) { calls++; return value; }
    public static String text(String value) { calls++; return value; }
    public static List<String> list(List<String> value) { calls++; return value; }
    public static Object objectValue(Object value) { calls++; events += value == null ? "N" : "V"; return value; }
    public static final class CountingList extends java.util.ArrayList<String> {
        public int iteratorCalls;
        public CountingList() { add("a"); add("bc"); }
        @Override public java.util.Iterator<String> iterator() { iteratorCalls++; return super.iterator(); }
    }
    public static final class EqualBox {
        public final int value;
        public int comparisons;
        public EqualBox(int value) { this.value = value; }
        @Override public boolean equals(Object other) { comparisons++; return other instanceof EqualBox && ((EqualBox) other).value == value; }
        @Override public int hashCode() { return value; }
    }
}
