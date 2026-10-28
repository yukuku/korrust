package sealednew;
public final class Probe {
    public static int observations;
    public static int marked;
    public static void observe() { observations++; }
    public static int mark(int value) { marked = value; return value; }
}
