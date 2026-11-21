import yuku.alkitab.base.util.QueryTokenizer;
import java.util.Arrays;

public class QueryCheck {
    private static void check(String input, String... expected) {
        String[] result = QueryTokenizer.tokenize(input);
        if (!Arrays.equals(result, expected)) throw new AssertionError(Arrays.toString(result));
    }
    public static void main(String[] args) {
        check(null);
        check("");
        check(" + ");
        check("One TWO", "one", "two");
        check("++Two", "+two");
        check("“My Word”", "+my word");
        if (!QueryTokenizer.isPlussedToken("+word") || QueryTokenizer.isPlussedToken("")) throw new AssertionError();
        if (!QueryTokenizer.tokenWithoutPlus("++two").equals("+two")) throw new AssertionError();
        var matchers = QueryTokenizer.matcherizeTokens(new String[]{"+man", "a.b"});
        if (matchers[0].reset("human").find() || !matchers[0].reset("Man!").find()) throw new AssertionError();
        if (matchers[1].reset("axb").find() || !matchers[1].reset("A.B").find()) throw new AssertionError();
        System.out.println("10 cases passed");
    }
}
