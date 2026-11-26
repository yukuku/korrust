package inherited.library;
public class JavaTypes {
    public static class Base {
        public static class Result {
            private final String value;
            public Result(String value) { this.value = value; }
            public String tag() { return value; }
            public static Result make(String value) { return new Result(value); }
        }
        public static class Token<T> {
            public final T value;
            public Token(T value) { this.value = value; }
        }
        public static class ArrayList { public static String tag() { return "member-list"; } }
        protected static class Protected { public static String tag() { return "protected"; } }
        static class PackageType { public static String tag() { return "package"; } }
        private static class Hidden {}
    }
    public static class Mid extends Base {}
    public static class Near extends Base {
        public static class Result extends Base.Result {
            public Result(String value) { super("near:" + value); }
        }
    }
    public static class PrivateHider extends Base { private static class Result {} }
    public static class Other { public static class Result {} }
}
