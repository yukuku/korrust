public class RegressionCheck { public static void main(String[] args) {
if (!"x".equals(Java_generic_methodKt.f(new fixtures.ForeignMethods()))) throw new AssertionError("generic instance method");
System.out.println("ok"); } }
