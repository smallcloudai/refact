import { useEffect } from "react";
import { selectAddressURL } from "../features/Config/configSlice";
import { useAppDispatch, useAppSelector } from "../hooks";
import { smallCloudApi } from "../services/smallcloud";

export const AddressURLSubscriber = () => {
  const dispatch = useAppDispatch();
  const addressURL = useAppSelector(selectAddressURL);

  useEffect(() => {
    dispatch(smallCloudApi.util.invalidateTags(["User"]));
  }, [dispatch, addressURL]);

  return null;
};
