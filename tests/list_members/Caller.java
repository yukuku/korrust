import java.util.ArrayList;
import java.util.List;
import slices.ConsumerKt;

public class Caller {
    public static void main(String[] args) {
        List<String> input = new ArrayList<>(List.of("x", "z", "x"));
        if (!ConsumerKt.read(input, 0, 2).equals(List.of("x", "z"))) throw new AssertionError();
        if (!ConsumerKt.element(input).equals("x")) throw new AssertionError();
        if (!ConsumerKt.named(input).equals(List.of("z"))) throw new AssertionError();
        if (!ConsumerKt.render(input).equals("[x, z, x]")) throw new AssertionError();
        if (!ConsumerKt.alternate(input).equals("[x]")) throw new AssertionError();
        if (ConsumerKt.where(input, "x") != 0 || ConsumerKt.last(input, "x") != 2) throw new AssertionError();
        ConsumerKt.writable(input).set(0, "changed");
        if (!input.get(1).equals("changed")) throw new AssertionError();
        try {
            ConsumerKt.read(input, -1, 1);
            throw new AssertionError();
        } catch (IndexOutOfBoundsException expected) {}
    }
}
