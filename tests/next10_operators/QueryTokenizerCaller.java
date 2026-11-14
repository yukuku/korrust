import java.util.Arrays;
import yuku.alkitab.base.util.QueryTokenizer;
public class QueryTokenizerCaller {
    private static void check(boolean condition) {
        if (!condition) throw new AssertionError();
    }
    public static void main(String[] args) {
        check(Arrays.equals(QueryTokenizer.tokenize(null), new String[0]));
        check(Arrays.equals(QueryTokenizer.tokenize(""), new String[0]));
        check(Arrays.equals(QueryTokenizer.tokenize("Hello WORLD"), new String[]{"hello", "world"}));
        check(Arrays.equals(QueryTokenizer.tokenize("\"Hello World\""), new String[]{"+hello world"}));
        check(Arrays.equals(QueryTokenizer.tokenize("+word"), new String[]{"+word"}));
        check(QueryTokenizer.isPlussedToken("+word"));
        check(!QueryTokenizer.isPlussedToken("word"));
        check(!QueryTokenizer.isPlussedToken(""));
        check(QueryTokenizer.tokenWithoutPlus("+word").equals("word"));
        check(QueryTokenizer.tokenWithoutPlus("word").equals("word"));
        check(QueryTokenizer.matcherizeTokens(new String[]{"+word"})[0].reset("word").find());
        check(!QueryTokenizer.matcherizeTokens(new String[]{"+word"})[0].reset("sword").find());
        check(QueryTokenizer.matcherizeTokens(new String[]{"word"})[0].reset("sword").find());
        System.out.println("OK:13");
    }
}
