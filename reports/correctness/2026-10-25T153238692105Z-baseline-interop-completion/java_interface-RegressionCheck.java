public class RegressionCheck { public static void main(String[] args) {
if (Java_interfaceKt.f(new fixtures.ForeignInterface() { public int value() { return 29; } }) != 29) throw new AssertionError("interface dispatch");
if (Java_interfaceKt.f(new fixtures.ForeignInterface() { public int value() { return -7; } }) != -7) throw new AssertionError("interface implementation");
System.out.println("ok"); } }
