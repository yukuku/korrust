import callbackconsumer.ConsumerKt;
import java.util.List;
public final class ConsumerCheck {
    public static void main(String[] args) throws Exception {
        Check.suspended(ConsumerKt::use, 3, List.of(9, 9, 9));
        Check.suspended(ConsumerKt::scalar, 1, 3);
        System.out.println("OK");
    }
}
