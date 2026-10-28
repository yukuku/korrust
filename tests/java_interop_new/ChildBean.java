package newinterop;

public class ChildBean extends Bean {
    @Override public String getTitle() { return "child:" + super.getTitle(); }
}
