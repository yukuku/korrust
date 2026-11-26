package model;
public class Outer {
    private final int value;
    public Outer(int value) { this.value = value; }
    public static int reads;
    public static Outer get() { ++reads; return new Outer(7); }
    public static Outer nil() { return null; }
    public static int arg() { return ++reads; }
    class Hidden {
        final int n;
        Hidden(int n) { this.n = n + value; }
    }
    public class Empty { public int number() { return 4; } }
    private class Secret {}
}
