public class Effects {
    public static String trace = "";
    public static boolean left() { trace += "L"; return false; }
    public static void right() { trace += "R"; }
    public static void accept(Runnable block) { block.run(); }
}
