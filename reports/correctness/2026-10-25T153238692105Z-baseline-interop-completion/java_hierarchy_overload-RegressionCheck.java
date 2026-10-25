public class RegressionCheck { public static void main(String[] args) {
if (Java_hierarchy_overloadKt.f(new fixtures.ChildMethods()) != 1) throw new AssertionError("inherited overload resolution");
System.out.println("ok"); } }
