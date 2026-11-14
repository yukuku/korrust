package next10.shorts;
public final class Effects {
    public static String trace = "";
    public static short value(int v) { trace += v; return (short)v; }
    public static short[] array(short[] a) { trace += "A"; return a; }
    public static int index() { trace += "I"; return 0; }
}
