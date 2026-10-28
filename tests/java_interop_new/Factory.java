package newinterop;

public final class Factory {
    public static Bean current = new ChildBean();
    public static int calls;
    public static Bean receiver() { calls++; Bean.log += "receiver;"; return current; }
    public static String argument() { Bean.log += "argument;"; return "new"; }
}
