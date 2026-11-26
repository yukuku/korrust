package deepnullable;
import java.util.*;
public final class Library {
 public static List<String> words() { return Arrays.asList((String)null); }
 public static String[] array() { return new String[]{null}; }
}
