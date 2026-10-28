package sourcegeneric;
public final class Probe {
    public static int calls;
    public static int next() { return ++calls; }
}
