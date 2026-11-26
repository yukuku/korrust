public class Caller {
    public static void main(String[] args) throws Exception {
        Counter counter = new Counter();
        TaskQueue queue = new TaskQueue(counter);
        if (queue.getSize() != 0 || counter.calls != 0) throw new AssertionError();
        queue.add("task");
        if (queue.getSize() != 1 || queue.getNext() != 1 || queue.getNext() != 2
                || ConsumerKt.getTop() != 42) throw new AssertionError();
        for (java.lang.reflect.Field f : TaskQueue.class.getDeclaredFields())
            if (f.getName().equals("size") || f.getName().equals("next")) throw new AssertionError(f);
    }
}
