package datarefs;
import java.util.*;
public class Driver {
    private static void check(boolean value) { if (!value) throw new AssertionError(); }
    static class ProbeSet extends LinkedHashSet<Integer> {
        int equalsCalls;
        int hashCalls;
        ProbeSet() { super(Arrays.asList(1, 2)); }
        public boolean equals(Object value) { equalsCalls++; return super.equals(value); }
        public int hashCode() { hashCalls++; return super.hashCode(); }
    }
    public static void main(String[] args) {
        ProbeSet ids = new ProbeSet();
        Map<String, String> names = new LinkedHashMap<>();
        names.put("alpha", "one"); names.put("empty", null);
        Crate<String> boxed = new Crate<>("same");
        Selection first = new Selection(ids, names, boxed, null);
        Selection equal = new Selection(new LinkedHashSet<>(ids), new LinkedHashMap<>(names), boxed, null);
        check(first.equals(first) && ids.equalsCalls == 0);
        check(first != equal && first.equals(equal) && equal.equals(first) && ids.equalsCalls == 1);
        check(!first.equals(null) && !first.equals("wrong"));
        check(!first.equals(new Selection(ids, names, new Crate<>("same"), null)));
        int hash = ((ids.hashCode() * 31 + names.hashCode()) * 31 + boxed.hashCode()) * 31;
        ids.hashCalls = 0;
        check(first.hashCode() == hash && equal.hashCode() == hash && ids.hashCalls == 1);
        check(first.toString().equals("Selection(ids=[1, 2], names={alpha=one, empty=null}, boxed=crate, optional=null)"));
        check(first.component1() == ids && first.component2() == names && first.component3() == boxed && first.component4() == null);
        Set<Integer> optional = new LinkedHashSet<>(Arrays.asList(5));
        Selection copied = first.copy(Collections.emptySet(), Collections.emptyMap(), boxed, optional);
        check(copied.getIds().isEmpty() && copied.getNames().isEmpty() && copied.getBoxed() == boxed && copied.getOptional() == optional);
        check(copied.hashCode() == boxed.hashCode() * 31 + optional.hashCode());
        check(copied.toString().equals("Selection(ids=[], names={}, boxed=crate, optional=[5])"));
        Selection cloned = ConsumerKt.defaultCopy(first);
        check(cloned != first && cloned.equals(first) && cloned.getIds() == ids && cloned.getNames() == names && cloned.getBoxed() == boxed);
        Selection empty = ConsumerKt.emptyCopy(first);
        check(empty.getIds().isEmpty() && empty.getNames() == names && empty.getBoxed() == boxed && empty.getOptional() == null && first.getIds() == ids);
        check(ConsumerKt.first(first) == ids && ConsumerKt.option(first) == null);
        check(ConsumerKt.create(ids, names, boxed).equals(first));
        try { new Selection(null, names, boxed, null); throw new AssertionError(); } catch (NullPointerException expected) {}
        try { first.copy(ids, null, boxed, null); throw new AssertionError(); } catch (NullPointerException expected) {}
        System.out.println("OK");
    }
}
