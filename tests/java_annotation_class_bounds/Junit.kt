package annotationboundsuses
import org.junit.Test
class Junit {
    @Test fun defaults() {}
    @Test(expected = IllegalArgumentException::class, timeout = 12L) fun explicit() {}
}
