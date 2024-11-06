import { useState, useEffect, useMemo } from "react";
import { smallCloudApi } from "../services/smallcloud";
import { useGetUser } from "./useGetUser";

export function useGetUserSurvey() {
  const userData = useGetUser();

  const [open, setOpen] = useState(false);

  const shouldSkip = useMemo(() => {
    return (
      userData.data === undefined ||
      userData.data.retcode !== "OK" ||
      userData.data.questionnaire !== false
    );
  }, [userData.data]);

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
    setOpen,
    questionRequest,
    postSurvey,
    postSurveyResult,
  };
}
