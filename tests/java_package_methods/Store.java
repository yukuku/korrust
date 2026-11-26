package app;
public class Store {
    static int next(int i) { return i + 3; }
    int read(int i) { return i * 2; }
    private static int hidden() { return 3; }
}
