import branches.ConsumerKt;
import branches.Steps;

public class Caller {
    public static void main(String[] args) {
        String[] answers = {"negative", "minus one", "zero", "one", "many"};
        for (int i = 0; i < answers.length; ++i) {
            if (!answers[i].equals(ConsumerKt.nested(i - 2))) throw new AssertionError(i);
            if (Steps.cleanups != i + 1) throw new AssertionError("finally");
        }
        if (ConsumerKt.numbers(-1) != 11 || ConsumerKt.numbers(0) != 7 || ConsumerKt.numbers(1) != 9)
            throw new AssertionError();
    }
}
