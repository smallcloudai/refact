from typing import List

__all__ = ['EtaTracker']


class EtaTracker:
    def __init__(self, total_tasks: int):
        self.total_tasks = total_tasks
        self.observations: List[float] = []

    def append(self, time: float):
        assert len(self.observations) < self.total_tasks, "EtaTracker is full"
        self.observations.append(time)

    def eta(self) -> float:
        return self.average_time() * (self.total_tasks - len(self.observations))

    def average_time(self, window_size: int = 5) -> float:
        def _remove_outliers(data):
            q1 = sorted(data)[int(len(data) * 0.25)]
            q3 = sorted(data)[int(len(data) * 0.75)]
            iqr = q3 - q1
            lower_bound = q1 - 1.5 * iqr
            upper_bound = q3 + 1.5 * iqr
            return [x for x in data if lower_bound <= x <= upper_bound]

        def _running_avg(data, window_size):
            return [sum(data[i:i + window_size]) / window_size
                    for i in range(len(data) - window_size + 1)]

        observations = _remove_outliers(self.observations)
        if len(observations) > (window_size * 2):
            observations = _running_avg(observations, window_size=window_size)
        return sum(observations) / len(observations)
