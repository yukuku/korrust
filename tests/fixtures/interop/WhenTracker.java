package fixtures;
public final class WhenTracker {
    public String trace = "";
    private final int subject;
    private final int first;
    private final int second;
    private final int third;
    public WhenTracker(int subject, int first, int second, int third) {
        this.subject = subject;
        this.first = first;
        this.second = second;
        this.third = third;
    }
    public int subject() { trace += "S"; return subject; }
    public int first() { trace += "1"; return first; }
    public int second() { trace += "2"; return second; }
    public int third() { trace += "3"; return third; }
}
