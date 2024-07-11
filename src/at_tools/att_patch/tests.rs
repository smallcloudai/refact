#[cfg(test)]
mod tests {
    use crate::at_tools::att_patch::diff_formats::UnifiedDiffFormat;

    use super::*;

    #[tokio::test]
    async fn test_parse_diff_message() {
        let input = r#"Let's break down the changes needed to implement the task:

1. **Track the last bounding box position for each track:**
   - Add a new dictionary `_last_bboxes` to store the last bounding box positions.
   - Initialize this dictionary in the constructor.

2. **Check for duplicates before emitting an event:**
   - Add a method `_is_significant_movement` to check if the current bounding box has moved significantly compared to the last recorded position.

3. **Emit unique events only if the bounding box has moved significantly:**
   - Update the `_handle_abandoned_item` method to use `_is_significant_movement` before emitting an event.
   - Update the last bounding box position after emitting an event.

Here is the unified diff format for the changes:

```diff
--- /home/svakhreev/projects/dssl/abandonment_prototype/abandonment/nodes/event_emitter_node.py
+++ /home/svakhreev/projects/dssl/abandonment_prototype/abandonment/nodes/event_emitter_node.py
@@ ... @@
    def __init__(self, circle_radius: float, abandonment_frames: int, max_age: int, moved_frames: int,  **kwargs):
        ProcessConsumerProducer.__init__(self, **kwargs)
        self._circle_radius = circle_radius
        self._abandonment_frames = abandonment_frames
        self._max_age = max_age
        self._moved_frames = moved_frames
        self._abandonment_items = {}
+        self._last_bboxes = {}  # Track last bounding box positions

@@ ... @@
    def _handle_abandoned_item(self, item: Prediction, message: Message) -> None:
        if item.track_idx not in self._abandonment_items:
            self._abandonment_items[item.track_idx] = AbandonmentItem(prediction=item)
        else:
            self._abandonment_items[item.track_idx].moved_count = 0
            frames_diff = item.frame_idx - self._abandonment_items[item.track_idx].prediction.frame_idx
            if frames_diff > self._abandonment_frames and not self._abandonment_items[item.track_idx].is_sent:
+                if self._is_significant_movement(item):
                    event = Event(prediction=self._abandonment_items[item.track_idx].prediction, event_type=EventType.DIST_2)
                    message.data.events.append(event)
                    self._abandonment_items[item.track_idx].is_sent = True
+                    self._last_bboxes[item.track_idx] = item.bbox.center_xywh()[:2]  # Update last bbox position

+    def _is_significant_movement(self, item: Prediction) -> bool:
+        if item.track_idx not in self._last_bboxes:
+            return True
+        last_x, last_y = self._last_bboxes[item.track_idx]
+        current_x, current_y = item.bbox.center_xywh()[:2]
+        distance = math.sqrt((last_x - current_x) ** 2 + (last_y - current_y) ** 2)
+        return distance > self._circle_radius / 2  # Consider significant if moved more than half the radius
```

Explanation of changes:
1. **Constructor Update:**
   - Added `self._last_bboxes = {}` to track the last bounding box positions. (`+`)

2. **_handle_abandoned_item Method Update:**
   - Added a check using `_is_significant_movement(item)` before emitting an event. (`+`)
   - Updated the last bounding box position after emitting an event. (`+`)

3. **New Method _is_significant_movement:**
   - Added a new method `_is_significant_movement` to check if the current bounding box has moved significantly compared to the last recorded position. (`+`)

This diff should be applied to the file `/home/svakhreev/projects/dssl/abandonment_prototype/abandonment/nodes/event_emitter_node.py`. The changes ensure that duplicate events are filtered out based on their relative position using the last bounding box in the track."#;
        let result = UnifiedDiffFormat::parse_message(input).await.unwrap();
        info!("result: {:?}", result);
    }
}
