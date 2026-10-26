package compatibility.declarations;
public final class Effects {
    public static String events = "";
    public static void mark(String value) { events += value; }
    public static void reset() { events = ""; }
}
