import java.util.*;
import yuku.alkitab.base.util.AlphanumComparator;
public final class AppCalls {
    public static void main(String[] args) {
        Comparator<String> comparator = new AlphanumComparator();
        List<String> codes = new ArrayList<>(Arrays.asList("10", "a10", "9", "1b", "100", "2", "1a", "a2", "1"));
        codes.sort(comparator);
        if (!codes.equals(Arrays.asList("1", "1a", "1b", "2", "9", "10", "100", "a2", "a10"))) throw new AssertionError(codes);
        if (comparator.compare("007", "7") >= 0 || comparator.compare("\ud800", "\ud801") != -1 || comparator.compare("\u0663", "5") != 1582) throw new AssertionError();
        try { ((Comparator) comparator).compare(1,"x"); throw new AssertionError(); } catch (ClassCastException expected) {}
        try { comparator.compare(null,"x"); throw new AssertionError(); } catch (NullPointerException expected) {}
        System.out.println("OK");
    }
}
