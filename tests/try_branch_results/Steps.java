package branches;
public class Steps {
    public static int cleanups;
    public static boolean checked(int n) {
        if (n < 0) throw new IllegalStateException();
        return n == 0;
    }
}
