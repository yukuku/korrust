package ten.trace;

public final class Trace {
    public static String trace = "";
    public static String value = "second";
    public static boolean firstCondition = false;
    public static boolean secondCondition = true;
    public void markReceiver() { trace += "R"; }
    public String argument() { trace += "A"; return "x"; }
    public String fallback() { trace += "F"; return "fallback"; }
    public String subject() { trace += "S"; return value; }
    public String first() { trace += "1"; return "first"; }
    public String second() { trace += "2"; return "second"; }
    public String matched() { trace += "M"; return "matched"; }
    public boolean firstCondition() { trace += "a"; return firstCondition; }
    public boolean secondCondition() { trace += "b"; return secondCondition; }
}
