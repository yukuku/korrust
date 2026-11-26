import annotationfixture.*;
import java.util.Arrays;
public class Caller {
    static void expect(String[] actual, String... expected) {
        if (!Arrays.equals(actual, expected)) throw new AssertionError(Arrays.toString(actual));
    }
    public static void main(String[] args) {
        expect(Empty.class.getAnnotation(Mark.class).value());
        Mark mark = Many.class.getAnnotation(Mark.class);
        if (!"ok".equals(mark.label())) throw new AssertionError();
        expect(mark.value(), "a", "b");
        expect(Spread.class.getAnnotation(Mark.class).value(), "first", "second", "third");
        expect(Named.class.getAnnotation(Mark.class).value(), "named");
        expect(Default.class.getAnnotation(Preset.class).value(), "fallback");
    }
}
