package defaults.mocking;

public class MockKCalls {
    public static void main(String[] args) {
        if (MockKSmokeKt.relaxed() != 17) throw new AssertionError("relaxed/default block");
        if (MockKSmokeKt.strict() != 19) throw new AssertionError("strict/default flags");
        System.out.println("mockk-inline-defaults-ok");
    }
}
