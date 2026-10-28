package visibilityprobe;
public final class Effects {
 public static String trace = "";
 public static boolean failValue;
 public static final RuntimeException FAILURE = new RuntimeException("rhs");
 public static void mark(String value) { trace += value; }
 public static int value(int value) { trace += "V"; if (failValue) throw FAILURE; return value; }
}
