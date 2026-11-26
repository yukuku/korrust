package nullable;
public final class Library {
    public String field = null;
    public static String echo(String value) { return value; }
    @javax.annotation.Nonnull public static String required(@javax.annotation.Nonnull String value) { return value; }
}
