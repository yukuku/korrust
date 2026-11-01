package callee;
public class Arena {
    public static String trace = "";
    public static class Ambiguous { public Ambiguous() {} }
    public static String Ambiguous() { return "method"; }
    public static abstract class AbstractClash { public AbstractClash() {} }
    public static String AbstractClash() { return "method"; }
    public static class CtorOnly { public CtorOnly() { trace += "C"; } }
    public static String CtorOnly(int value) { trace += "c"; return "arity"; }
    public static class MethodOnly { public int value; public MethodOnly(int value) { this.value = value; trace += "n"; } }
    public static String MethodOnly() { trace += "m"; return "method"; }
    public static class StringCtor { public String value; public StringCtor(String value) { this.value = value; trace += "S"; } }
    public static String StringCtor(Object value) { trace += "wrong"; return "wrong"; }
    public static class ObjectCtor { public ObjectCtor(Object value) { trace += "wrong"; } }
    public static String ObjectCtor(String value) { trace += "M"; return "string-method"; }
    public static class PrivateCtor { private PrivateCtor() {} }
    public static String PrivateCtor() { trace += "P"; return "private"; }
    public class InnerCtor { public InnerCtor() {} }
    public static String InnerCtor() { trace += "I"; return "inner"; }
    private static class Hidden { public Hidden() {} }
    public static String Hidden() { trace += "H"; return "hidden"; }
    public static abstract class AbstractMethod { public AbstractMethod(int value) {} }
    public static String AbstractMethod() { trace += "B"; return "abstract"; }
}
