public class Caller {
    public static void main(String[] args) {
        if (!ConsumerKt.text().equals("text") || !ConsumerKt.sequence().equals("letters")) throw new AssertionError();
    }
}
