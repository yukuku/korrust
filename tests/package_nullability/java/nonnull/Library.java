package nonnull;
import java.util.*;
public final class Library {
    public String field = "field";
    public String brokenField = null;
    @javax.annotation.Nullable public String maybeField = null;
    public static String echo(String value) { return value; }
    public static String broken() { return null; }
    public static final class Node { public int touch(int value) { return value; } }
    public static Node brokenNode() { return null; }
    public static String[] brokenArray() { return null; }
    public static List<String> brokenList() { return null; }
    public static List<String> words() { return Arrays.asList("one", "two"); }
    public static String[] array() { return new String[]{"three"}; }
    @javax.annotation.Nullable public static String maybe(@javax.annotation.Nullable String value) { return value; }
    public static int length(String value) { return value.length(); }
    public static <T> T identity(T value) { return value; }
    @qualifiers.Required public static String localDefault(String value) { return value; }
}
