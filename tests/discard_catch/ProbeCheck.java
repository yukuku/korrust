public class ProbeCheck {
    public static void main(String[] args) {
        if (ProbeKt.probe(0) != 11 || ProbeKt.probe(1) != 21 || ProbeKt.probe(2) != 31) {
            throw new AssertionError();
        }
    }
}
