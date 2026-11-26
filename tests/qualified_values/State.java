package qv.p;

public class State {
    public static int count;
    public static final int FIXED = 9;
    public static final Root ROOT = new Root();
    public static int add(int value) { count += value; return count; }
}
