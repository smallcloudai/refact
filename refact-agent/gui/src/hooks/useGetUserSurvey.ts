import { useState, useEffect, useMemo, useCallback } from "react";
import { smallCloudApi } from "../services/smallcloud";
import {
  userSurveyWasAskedMoreThanADayAgo,
  setLastAsked,
} from "../features/UserSurvey/userSurveySlice";
import { useGetUser } from "./useGetUser";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";

export function useGetUserSurvey() {
  const userData = useGetUser();
  const askedMoreThanADayAgo = useAppSelector(
    userSurveyWasAskedMoreThanADayAgo,
  );

  const dispatch = useAppDispatch();

  const [open, setOpen] = useState(false);

  const shouldSkip = useMemo(() => {
    return (
      userData.data === undefined ||
      userData.data.retcode !== "OK" ||
      userData.data.questionnaire !== false ||
      !askedMoreThanADayAgo
    );
  }, [userData.data, askedMoreThanADayAgo]);

  const handleOpenChange = useCallback(
    (value: boolean) => {
      if (!value) {
        dispatch(setLastAsked());
      }
      setOpen(value);
    },
    [dispatch],
  );

  const questionRequest = smallCloudApi.useGetSurveyQuery(undefined, {
    skip: shouldSkip,
  });

  const [postSurvey, postSurveyResult] = smallCloudApi.usePostSurveyMutation();

  useEffect(() => {
    if (questionRequest.data && postSurveyResult.isUninitialized) {
      setOpen(true);
    }
  }, [postSurveyResult.isUninitialized, questionRequest.data]);

  return {
    open,
    handleOpenChange,
    questionRequest,
    postSurvey,
    postSurveyResult,
  };
}
