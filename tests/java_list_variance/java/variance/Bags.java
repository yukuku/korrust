package variance;
import java.util.*;
public class Bags {
    public List<CharSequence> field;
    public static int calls;
    public List<CharSequence> getValue() { return field; }
    public void setValue(List<CharSequence> value) { field = value; }
    public int readSize() { return field.size(); }
    public static int accept(List<CharSequence> value) { return value.size(); }
    public static int same(List<String> value) { return value.size(); }
    public static int nullable(List<CharSequence> value) { return value == null ? -1 : value.size(); }
    public static int choose(List<CharSequence> value) { return 1; }
    public static int choose(Object value) { return 2; }
    public int select(List<CharSequence> value) { return 1; }
    public int select(Object value) { return 2; }
    public static List<String> strings() { return new ArrayList<>(); }
    public static List<String> next() { calls++; return strings(); }
}
