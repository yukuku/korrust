package annotationboundscheck;
import annotationboundsuses.Junit;
import org.junit.Test;
public class JunitCheck {
    public static void main(String[] args) throws Exception {
        Test defaults = Junit.class.getDeclaredMethod("defaults").getAnnotation(Test.class);
        Test explicit = Junit.class.getDeclaredMethod("explicit").getAnnotation(Test.class);
        if (defaults.expected() != Test.None.class || defaults.timeout() != 0L) throw new AssertionError("defaults");
        if (explicit.expected() != IllegalArgumentException.class || explicit.timeout() != 12L) throw new AssertionError("explicit");
        System.out.println("OK:junit");
    }
}
