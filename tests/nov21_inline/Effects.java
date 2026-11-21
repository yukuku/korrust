package consumer;
public final class Effects {
  public static String log = "";
  public static int size() { log += "N"; return 3; }
  public static int failedSize() { log += "F"; throw new IllegalArgumentException("size-failure"); }
  public static Object next() {
    log += "E";
    return new Object() { public String toString() { log += "S"; return "value"; } };
  }
  public static Object broken() {
    log += "B";
    return new Object() { public String toString() { log += "X"; throw new IllegalStateException("string-failure"); } };
  }
}
