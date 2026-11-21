package baseline
import java.util.regex.Pattern
fun matchers(): Array<java.util.regex.Matcher> = Array(1) { Pattern.compile("a").matcher("") }
