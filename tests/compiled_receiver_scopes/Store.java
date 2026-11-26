package receiverlib;

import java.util.*;

public class Store {
    public List<String> names;
    public List<List<String>> nested;
    public Map<String, Integer> counts;
    public List<? extends CharSequence> projected;
    public List<?> star;
    public final List<String> fixed = Collections.emptyList();
    public static List<String> global;
    public static int trace;
    public static Store last;

    public static Store make() {
        trace = trace * 10 + 1;
        last = new Store();
        return last;
    }
    public static List<String> data() {
        trace = trace * 10 + 2;
        return Collections.singletonList("ordered");
    }
    public static Store failMake() {
        trace = trace * 10 + 1;
        throw new IllegalStateException("receiver");
    }
    public static List<String> failData() {
        trace = trace * 10 + 2;
        throw new IllegalArgumentException("value");
    }
}
