import java.util.*;
import org.junit.runner.*;
import org.junit.runner.manipulation.Filter;
import org.junit.runner.notification.Failure;
public class RunMethods {
    public static void main(String[] args) throws Exception {
        List<String> methods = Arrays.asList(args).subList(1,args.length);
        Filter filter = new Filter() {
            public String describe() { return methods.toString(); }
            public boolean shouldRun(Description d) {
                if (d.isTest()) return methods.contains(d.getMethodName());
                return d.getChildren().stream().anyMatch(this::shouldRun);
            }
        };
        Result r = new JUnitCore().run(Request.aClass(Class.forName(args[0])).filterWith(filter));
        for (Failure f : r.getFailures()) System.out.println(f.getTrace());
        System.out.println("JUnit ran " + r.getRunCount() + " tests, failures " + r.getFailureCount());
        System.exit(r.wasSuccessful() && r.getRunCount() == methods.size() ? 0 : 1);
    }
}
