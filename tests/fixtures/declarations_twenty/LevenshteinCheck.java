import yuku.alkitab.base.util.Levenshtein;
public final class LevenshteinCheck {
    public static void main(String[] args) {
        String[] left = {"", "", "abc", "a", "a", "ab", "abc", "a", "\ud83d\ude00"};
        String[] right = {"", "abc", "", "a", "b", "ab", "abc", "ba", "\ud83d\ude00"};
        int[] expected = {0, 300, 1500, 2, 400, 6, 12, 104, 6};
        for (int i = 0; i < left.length; i++) {
            int actual = Levenshtein.distance(left[i], right[i]);
            if (actual != expected[i]) throw new AssertionError(i + ": " + actual);
        }
        System.out.println("{\"checks\":" + expected.length + "}");
    }
}
