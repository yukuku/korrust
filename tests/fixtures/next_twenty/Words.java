package next.support;
import java.util.Arrays;
import java.util.Iterator;
public final class Words implements Iterable<String> {
    public int iteratorCalls;
    @Override public Iterator<String> iterator() { iteratorCalls++; return Arrays.asList("a", "bc").iterator(); }
}
