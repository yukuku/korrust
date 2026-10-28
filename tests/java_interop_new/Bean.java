package newinterop;

public class Bean {
    public static final int CONSTANT = 17;
    public static int counter = 1;
    public int exposed = 3;
    private String title = "base";
    private boolean ready;
    public static String log = "";

    public static String choose(int value) { return "i" + value; }
    public static String choose(String value) { return "s" + value; }
    public static String nullableText() { return null; }
    public static int sum(int... values) {
        int result = 0;
        for (int value : values) result += value;
        return result;
    }

    public String getTitle() { log += "get;"; return title; }
    public Bean setTitle(String title) { log += "set(" + title + ");"; this.title = title; return this; }
    public boolean isReady() { return ready; }
    public void setReady(boolean ready) { this.ready = ready; }
    public String getURLValue() { return "url"; }
    public int getExposed() { return 99; }
    public String getReadOnly() { return "read"; }
    public void setOnly(String value) { title = value; }
    protected String getProtectedValue() { return "protected"; }
    private String getPrivateValue() { return "private"; }
}
