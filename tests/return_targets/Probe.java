import java.io.Closeable;
public final class Probe implements Closeable {
 public static String trace = "";
 public static void mark(String text) { trace += text; }
 @Override public void close() { mark("C"); }
}
